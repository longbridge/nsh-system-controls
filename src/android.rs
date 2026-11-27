//! The controls on Android: the framework's views of the crate's Java
//! library (`android/java`, packaged as `android/nsh-system-controls.jar`
//! through `[package.metadata.nsh] android.libraries`), made for each
//! element by the factories registered on the views of the hosts nsh
//! builds for the app.
//!
//! Each element's Java control takes its props as JSON text
//! (`setProp(name, json)`) and tells its app through one listener, a
//! `java.util.function.BiConsumer<String, String>` implemented in Rust
//! ([`nsh::proxy`]): `size` with `[width, height]` in dp, any other name an
//! event with its JSON data (`Bridge.java`).

use std::rc::Rc;

use nsh::{
    ComponentContext, ComponentFactory, Components, NativeComponent,
    jni::{
        JNIEnv,
        objects::{GlobalRef, JClass, JObject, JString, JValue},
    },
    proxy::{self, Proxy},
};
use nsh_core::JsonValue;

use crate::{DATE_TIME_PICKER_COMPONENT, SWITCH_COMPONENT};

/// The Java control of each component, by binary name.
const SWITCH_CLASS: &str = "com.longbridge.nsh.systemcontrols.SwitchComponent";
const DATE_TIME_PICKER_CLASS: &str = "com.longbridge.nsh.systemcontrols.DateTimePickerComponent";

/// Registers the controls on `components`.
pub(crate) fn register(components: &Components) {
    components.register(SWITCH_COMPONENT, factory(SWITCH_CLASS));
    components.register(DATE_TIME_PICKER_COMPONENT, factory(DATE_TIME_PICKER_CLASS));
}

/// Makes the Java control `class` of each element; an empty view, logged,
/// where it cannot (the crate's Java library missing from the package).
fn factory(class: &'static str) -> ComponentFactory {
    Rc::new(move |context: ComponentContext| {
        match context.with_env(|env| JavaControl::new(env, class, &context)) {
            Ok(control) => Box::new(control) as Box<dyn NativeComponent>,
            Err(error) => {
                log::error!(
                    "nsh-system-controls: could not make `{class}` ({error}); is the crate's Java \
                     library packaged?"
                );
                Box::new(Empty::new(&context)) as Box<dyn NativeComponent>
            }
        }
    })
}

/// An element's Java control.
struct JavaControl {
    control: GlobalRef,
    view: GlobalRef,
    /// What the control tells its app through; dropped with the element,
    /// after which a late call (a dialog's pick) does nothing.
    _listener: Proxy,
}

impl JavaControl {
    fn new(
        env: &mut JNIEnv<'_>,
        class: &str,
        context: &ComponentContext,
    ) -> nsh::jni::errors::Result<Self> {
        let android = context.android_context();
        // By the class loader of the view's context, the app's, which loads
        // the crate's Java library from the package; JNI's `FindClass`
        // would take the loader of whatever called into Rust.
        let loader = env
            .call_method(android, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let name = env.new_string(class)?;
        let class = env
            .call_method(
                &loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let tell = context.clone();
        let listener = proxy::implement(
            env,
            &["java.util.function.BiConsumer"],
            move |env, method, args| {
                if method == "accept" {
                    let name = env.get_object_array_element(args, 0)?;
                    let name = string(env, name)?;
                    let json = env.get_object_array_element(args, 1)?;
                    let json = string(env, json)?;
                    told(&tell, &name, &json);
                }
                Ok(None)
            },
        )?;
        let control = env.new_object(
            JClass::from(class),
            "(Landroid/content/Context;Ljava/util/function/BiConsumer;)V",
            &[
                JValue::Object(android.as_obj()),
                JValue::Object(listener.as_obj()),
            ],
        )?;
        let view = env
            .call_method(&control, "view", "()Landroid/view/View;", &[])?
            .l()?;
        Ok(Self {
            control: env.new_global_ref(control)?,
            view: env.new_global_ref(view)?,
            _listener: listener,
        })
    }
}

/// A Java string argument as Rust's (`""` for null).
fn string(env: &mut JNIEnv<'_>, object: JObject<'_>) -> nsh::jni::errors::Result<String> {
    if object.is_null() {
        return Ok(String::new());
    }
    Ok(env.get_string(&JString::from(object))?.into())
}

/// What a control told its app: its own size (`size`), or an event.
fn told(context: &ComponentContext, name: &str, json: &str) {
    let data = serde_json::from_str(json).unwrap_or(JsonValue::Null);
    if name == "size" {
        let side = |i: usize| data.get(i).and_then(JsonValue::as_f64).unwrap_or(0.0) as f32;
        context.set_size(side(0), side(1));
    } else {
        context.emit(name, data);
    }
}

impl NativeComponent for JavaControl {
    fn view(&self) -> &GlobalRef {
        &self.view
    }

    fn set_prop(&mut self, env: &mut JNIEnv<'_>, name: &str, value: Option<&JsonValue>) {
        let json = value.map(JsonValue::to_string);
        let set = env.with_local_frame(4, |env| -> nsh::jni::errors::Result<()> {
            let name = env.new_string(name)?;
            let json = match &json {
                Some(json) => JObject::from(env.new_string(json)?),
                None => JObject::null(),
            };
            env.call_method(
                &self.control,
                "setProp",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[JValue::Object(&name), JValue::Object(&json)],
            )?;
            Ok(())
        });
        if let Err(error) = set {
            log::warn!("nsh-system-controls: prop `{name}` was not set: {error}");
        }
    }
}

/// An empty view, for an element whose control could not be made.
struct Empty {
    view: GlobalRef,
}

impl Empty {
    fn new(context: &ComponentContext) -> Self {
        let view = context
            .with_env(|env| {
                let view = env.new_object(
                    "android/view/View",
                    "(Landroid/content/Context;)V",
                    &[JValue::Object(context.android_context().as_obj())],
                )?;
                env.new_global_ref(view)
            })
            // The element needs a view; one the framework cannot make
            // leaves nothing to show.
            .expect("nsh-system-controls: an android.view.View");
        Self { view }
    }
}

impl NativeComponent for Empty {
    fn view(&self) -> &GlobalRef {
        &self.view
    }

    fn set_prop(&mut self, _env: &mut JNIEnv<'_>, _name: &str, _value: Option<&JsonValue>) {}
}
