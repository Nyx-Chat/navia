use jni::JNIEnv;
use jni::objects::{JByteBuffer, JClass, JObject, JString};
use jni::sys::{jbyteArray, jlong};

use crate::messaging::{DidComInterface, DIDCommMessage};

// Helper to convert JNI string to Rust string
fn jstring_to_string(env: &mut JNIEnv, jstr: &JString) -> Result<String, jni::errors::Error> {
    env.get_string(jstr).map(|s| s.into())
}

// Helper to convert Rust string to JNI byte array
fn string_to_jbytearray(env: &mut JNIEnv, string: String) -> Result<jbyteArray, jni::errors::Error> {
    let bytes = string.as_bytes();
    let array = env.new_byte_array(bytes.len() as i32)?;
    env.set_byte_array_region(&array, 0, &bytes.iter().map(|&b| b as i8).collect::<Vec<i8>>())?;
    Ok(array.into_raw())
}

// Zero-copy pack using DirectByteBuffer
#[no_mangle]
pub extern "system" fn Java_com_nyx_kotlin_data_navia_NaviaNative_packDirectBuffer(
    mut env: JNIEnv,
    _class: JClass,
    instance_ptr: jlong,
    msg_buffer: JObject,     // DirectByteBuffer with message content
    msg_id: JString,
    msg_type: JString,
    from: JString,
    to: JString,
) -> jbyteArray {
    // Get the DidComInterface instance from the pointer
    let interface = unsafe { &*(instance_ptr as *const DidComInterface) };
    
    // Get message content from DirectByteBuffer
    let msg_buffer = unsafe { JByteBuffer::from_raw(msg_buffer.into_raw()) };
    let msg_content = match env.get_direct_buffer_address(&msg_buffer) {
        Ok(addr) => {
            let len = match env.get_direct_buffer_capacity(&msg_buffer) {
                Ok(len) => len,
                Err(_) => return std::ptr::null_mut(),
            };
            
            let slice = unsafe {
                std::slice::from_raw_parts(addr, len)
            };
            
            match std::str::from_utf8(slice) {
                Ok(s) => s.to_string(),
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(_) => return std::ptr::null_mut(),
    };
    
    // Convert JStrings to Rust strings
    let msg_id = match jstring_to_string(&mut env, &msg_id) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let msg_type = match jstring_to_string(&mut env, &msg_type) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let from = match jstring_to_string(&mut env, &from) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let to = match jstring_to_string(&mut env, &to) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    // Create DIDCommMessage
    let msg = DIDCommMessage {
        id: msg_id,
        msg_type,
        body: msg_content,
        from: Some(from.clone()),
        to: vec![to.clone()],
    };
    
    // Use the runtime to execute async pack
    let result = interface.runtime.block_on(async {
        interface.pack(msg, from, to).await
    });
    
    match result {
        Ok(packed) => {
            match string_to_jbytearray(&mut env, packed) {
                Ok(array) => array,
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// Zero-copy unpack using DirectByteBuffer
#[no_mangle]
pub extern "system" fn Java_com_nyx_kotlin_data_navia_NaviaNative_unpackDirectBuffer(
    mut env: JNIEnv,
    _class: JClass,
    instance_ptr: jlong,
    msg_buffer: JObject,     // DirectByteBuffer with packed message
) -> jbyteArray {
    // Get the DidComInterface instance from the pointer
    let interface = unsafe { &*(instance_ptr as *const DidComInterface) };
    
    // Get packed message from DirectByteBuffer
    let msg_buffer = unsafe { JByteBuffer::from_raw(msg_buffer.into_raw()) };
    let packed_msg = match env.get_direct_buffer_address(&msg_buffer) {
        Ok(addr) => {
            let len = match env.get_direct_buffer_capacity(&msg_buffer) {
                Ok(len) => len,
                Err(_) => return std::ptr::null_mut(),
            };
            
            let slice = unsafe {
                std::slice::from_raw_parts(addr, len)
            };
            
            match std::str::from_utf8(slice) {
                Ok(s) => s.to_string(),
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(_) => return std::ptr::null_mut(),
    };
    
    // Use the runtime to execute async unpack
    let result = interface.runtime.block_on(async {
        interface.unpack(packed_msg).await
    });
    
    match result {
        Ok(message) => {
            // Convert DIDCommMessage to JSON for compatibility
            let json = serde_json::json!({
                "id": message.id,
                "type": message.msg_type,
                "body": serde_json::json!({
                    "content": message.body
                }),
                "from": message.from,
                "to": message.to,
            });
            
            match string_to_jbytearray(&mut env, json.to_string()) {
                Ok(array) => array,
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// Get native pointer for the DidComInterface instance
#[no_mangle]
pub extern "system" fn Java_com_nyx_kotlin_data_navia_NaviaNative_createNativeInstance(
    mut _env: JNIEnv,
    _class: JClass,
    db_path: JString,
) -> jlong {
    let path = match jstring_to_string(&mut _env, &db_path) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    let interface = Box::new(DidComInterface::new(path));
    Box::into_raw(interface) as jlong
}

// Clean up native instance
#[no_mangle]
pub extern "system" fn Java_com_nyx_kotlin_data_navia_NaviaNative_destroyNativeInstance(
    _env: JNIEnv,
    _class: JClass,
    instance_ptr: jlong,
) {
    if instance_ptr != 0 {
        unsafe {
            let _ = Box::from_raw(instance_ptr as *mut DidComInterface);
            // Box will be dropped, cleaning up the instance
        }
    }
}