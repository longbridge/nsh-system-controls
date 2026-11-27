package com.longbridge.nsh.systemcontrols;

import android.app.Activity;
import android.content.Context;
import android.content.ContextWrapper;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.function.BiConsumer;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * What the controls share with the crate's Rust, which makes them for their elements ({@code src/android.rs}):
 * props arrive as JSON text, and what a control tells its app leaves through one listener, {@code (name, json)}:
 * {@code size} with {@code [width, height]} in dp for the control's own size, any other name an event its
 * handler gets.
 */
final class Bridge {
    private final BiConsumer<String, String> listener;

    Bridge(BiConsumer<String, String> listener) {
        this.listener = listener;
    }

    /** Emits {@code event}: the handler {@code on<Event>} gets {@code data} (a {@code Boolean}, a {@code Map}). */
    void emit(String event, Object data) {
        Object json = data instanceof Map ? new JSONObject((Map<?, ?>) data) : data;
        listener.accept(event, String.valueOf(json));
    }

    /** The control's own size in dp, used where its style leaves a dimension open. */
    void setSize(float width, float height) {
        listener.accept("size", "[" + width + "," + height + "]");
    }

    /**
     * A prop's JSON as the controls read it: {@code String}, {@code Number}, {@code Boolean}, {@code Map}, or null
     * for JSON {@code null}, a removed prop or text that is not JSON.
     */
    static Object parse(String json) {
        if (json == null) return null;
        try {
            return plain(new JSONTokener(json).nextValue());
        } catch (JSONException e) {
            return null;
        }
    }

    private static Object plain(Object value) throws JSONException {
        if (value == JSONObject.NULL) return null;
        if (value instanceof JSONObject) {
            JSONObject object = (JSONObject) value;
            Map<String, Object> map = new HashMap<>();
            for (Iterator<String> keys = object.keys(); keys.hasNext(); ) {
                String key = keys.next();
                map.put(key, plain(object.get(key)));
            }
            return map;
        }
        if (value instanceof JSONArray) {
            JSONArray array = (JSONArray) value;
            java.util.List<Object> list = new java.util.ArrayList<>(array.length());
            for (int i = 0; i < array.length(); i++) list.add(plain(array.get(i)));
            return list;
        }
        return value;
    }

    /**
     * Whether the activity {@code context} shows in is going: finishing, or going to be made again for a
     * configuration change. In NSH's own activities the context is a wrapper of the activity, which still holds the
     * going one while the views leave it.
     */
    static boolean activityGoing(Context context) {
        Context c = context;
        // An activity is a wrapper too: it is found before its base.
        while (c instanceof ContextWrapper) {
            if (c instanceof Activity) {
                Activity activity = (Activity) c;
                return activity.isChangingConfigurations() || activity.isFinishing() || activity.isDestroyed();
            }
            c = ((ContextWrapper) c).getBaseContext();
        }
        return false;
    }
}
