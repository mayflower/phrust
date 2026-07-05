//! Deterministic System V message queue compatibility slice.

use super::core::{argument_type_error, arity_error, assign_reference_arg, int_arg, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan, context::SysvMessage,
};
use crate::{
    ArrayKey, ClassEntry, ClassFlags, ObjectRef, PhpArray, PhpString, Value, normalize_class_name,
};

const QUEUE_CLASS: &str = "SysvMessageQueue";
const QUEUE_ID_PROPERTY: &str = "__sysvmsg_id";

const MSG_EAGAIN: i64 = libc::EAGAIN as i64;
const MSG_ENOMSG: i64 = libc::ENOMSG as i64;
const MSG_NOERROR: i64 = 0o10000;
const MSG_EXCEPT: i64 = 0o20000;
const E2BIG: i64 = libc::E2BIG as i64;

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "msg_get_queue",
        builtin_msg_get_queue,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("msg_send", builtin_msg_send, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "msg_receive",
        builtin_msg_receive,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "msg_remove_queue",
        builtin_msg_remove_queue,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "msg_stat_queue",
        builtin_msg_stat_queue,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "msg_set_queue",
        builtin_msg_set_queue,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "msg_queue_exists",
        builtin_msg_queue_exists,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_msg_get_queue(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_between("msg_get_queue", &args, 1, 2)?;
    let key = int_arg("msg_get_queue", &args[0])?;
    let permissions = optional_int("msg_get_queue", &args, 1, 0o666)?;
    let id = context.sysvmsg_state().get_queue(key, permissions);
    Ok(Value::Object(queue_object(id)))
}

fn builtin_msg_send(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_between("msg_send", &args, 3, 6)?;
    let queue_id = queue_id("msg_send", &args[0])?;
    let message_type = int_arg("msg_send", &args[1])?;
    if message_type <= 0 {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_SYSVMSG_TYPE",
            "msg_send(): Argument #2 ($message_type) must be greater than 0",
        ));
    }
    let serialize = optional_bool("msg_send", &args, 3, true)?;
    let blocking = optional_bool("msg_send", &args, 4, true)?;
    let payload = if serialize {
        crate::serialize(&args[2])
            .map_err(|error| BuiltinError::new("E_PHP_RUNTIME_SYSVMSG_SERIALIZE", error.message()))?
            .as_bytes()
            .to_vec()
    } else {
        message_scalar_bytes("msg_send", &args[2])?
    };

    if !blocking
        && context
            .sysvmsg_state()
            .queue(queue_id)
            .is_some_and(|queue| queue.byte_count() + payload.len() > queue.max_bytes() as usize)
    {
        assign_reference_arg(args.get(5), Value::Int(MSG_EAGAIN));
        return Ok(Value::Bool(false));
    }

    let sent = context
        .sysvmsg_state()
        .send(queue_id, SysvMessage::new(message_type, payload, serialize));
    if sent {
        assign_reference_arg(args.get(5), Value::Int(0));
    } else {
        assign_reference_arg(args.get(5), Value::Int(MSG_EAGAIN));
    }
    Ok(Value::Bool(sent))
}

fn builtin_msg_receive(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_between("msg_receive", &args, 5, 8)?;
    let queue_id = queue_id("msg_receive", &args[0])?;
    let desired_type = int_arg("msg_receive", &args[1])?;
    let max_size = int_arg("msg_receive", &args[3])?;
    let unserialize = optional_bool("msg_receive", &args, 5, true)?;
    let flags = optional_int("msg_receive", &args, 6, 0)?;
    let except = flags & MSG_EXCEPT != 0 && desired_type > 0;
    let Some(message) = context
        .sysvmsg_state()
        .receive(queue_id, desired_type, except)
    else {
        assign_reference_arg(args.get(7), Value::Int(MSG_ENOMSG));
        return Ok(Value::Bool(false));
    };

    if max_size >= 0 && message.payload().len() > max_size as usize {
        if flags & MSG_NOERROR == 0 {
            context.sysvmsg_state().send(queue_id, message);
            assign_reference_arg(args.get(7), Value::Int(E2BIG));
            return Ok(Value::Bool(false));
        }
        let truncated = SysvMessage::new(
            message.message_type(),
            message.payload()[..max_size as usize].to_vec(),
            message.is_serialized(),
        );
        return receive_message(args, truncated, unserialize);
    }

    receive_message(args, message, unserialize)
}

fn receive_message(args: Vec<Value>, message: SysvMessage, unserialize: bool) -> BuiltinResult {
    assign_reference_arg(args.get(2), Value::Int(message.message_type()));
    let value = if unserialize && message.is_serialized() {
        crate::unserialize(
            &PhpString::from_bytes(message.payload().to_vec()),
            crate::UnserializeOptions::default(),
        )
        .map_err(|error| BuiltinError::new("E_PHP_RUNTIME_SYSVMSG_UNSERIALIZE", error.message()))?
    } else {
        Value::string(message.payload().to_vec())
    };
    assign_reference_arg(args.get(4), value);
    assign_reference_arg(args.get(7), Value::Int(0));
    Ok(Value::Bool(true))
}

fn builtin_msg_remove_queue(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("msg_remove_queue", &args, 1)?;
    let queue_id = queue_id("msg_remove_queue", &args[0])?;
    Ok(Value::Bool(context.sysvmsg_state().remove_queue(queue_id)))
}

fn builtin_msg_stat_queue(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("msg_stat_queue", &args, 1)?;
    let queue_id = queue_id("msg_stat_queue", &args[0])?;
    let Some(queue) = context.sysvmsg_state().queue(queue_id) else {
        return Ok(Value::Bool(false));
    };
    let mut result = PhpArray::new();
    result.insert(string_key("msg_perm.key"), Value::Int(queue.key()));
    result.insert(string_key("msg_perm.mode"), Value::Int(queue.permissions()));
    result.insert(
        string_key("msg_qnum"),
        Value::Int(queue.message_count() as i64),
    );
    result.insert(string_key("msg_qbytes"), Value::Int(queue.max_bytes()));
    result.insert(
        string_key("msg_cbytes"),
        Value::Int(queue.byte_count() as i64),
    );
    Ok(Value::Array(result))
}

fn builtin_msg_set_queue(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("msg_set_queue", &args, 2)?;
    let queue_id = queue_id("msg_set_queue", &args[0])?;
    let Value::Array(data) = &args[1] else {
        return Err(argument_type_error(
            "msg_set_queue",
            "#2 ($data)",
            "array",
            &args[1],
        ));
    };
    let Some(queue) = context.sysvmsg_state().queue_mut(queue_id) else {
        return Ok(Value::Bool(false));
    };
    if let Some(value) = data.get(&string_key("msg_perm.mode")) {
        queue.set_permissions(int_arg("msg_set_queue", value)?);
    }
    if let Some(value) = data.get(&string_key("msg_qbytes")) {
        queue.set_max_bytes(int_arg("msg_set_queue", value)?);
    }
    Ok(Value::Bool(true))
}

fn builtin_msg_queue_exists(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("msg_queue_exists", &args, 1)?;
    let key = int_arg("msg_queue_exists", &args[0])?;
    Ok(Value::Bool(context.sysvmsg_state().queue_exists(key)))
}

fn expect_exact(name: &str, args: &[Value], expected: usize) -> Result<(), BuiltinError> {
    expect_between(name, args, expected, expected)
}

fn expect_between(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), BuiltinError> {
    if (min..=max).contains(&args.len()) {
        Ok(())
    } else {
        Err(arity_error(
            name,
            &format!("between {min} and {max} arguments"),
        ))
    }
}

fn optional_int(
    name: &str,
    args: &[Value],
    index: usize,
    default: i64,
) -> Result<i64, BuiltinError> {
    args.get(index)
        .map_or(Ok(default), |value| int_arg(name, value))
}

fn optional_bool(
    name: &str,
    args: &[Value],
    index: usize,
    default: bool,
) -> Result<bool, BuiltinError> {
    args.get(index).map_or(Ok(default), |value| {
        crate::convert::to_bool(value).map_err(|message| {
            BuiltinError::new("E_PHP_RUNTIME_BUILTIN_TYPE", format!("{name}(): {message}"))
        })
    })
}

fn message_scalar_bytes(name: &str, value: &Value) -> Result<Vec<u8>, BuiltinError> {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) | Value::Resource(_) => Err(
            argument_type_error(name, "#3 ($message)", "string|int|float|bool", value),
        ),
        _ => Ok(string_arg(name, value)?.as_bytes().to_vec()),
    }
}

fn queue_id(name: &str, value: &Value) -> Result<i64, BuiltinError> {
    let Value::Object(object) = value else {
        return Err(argument_type_error(name, "#1 ($queue)", QUEUE_CLASS, value));
    };
    if normalize_class_name(&object.class_name()) != "sysvmessagequeue" {
        return Err(argument_type_error(name, "#1 ($queue)", QUEUE_CLASS, value));
    }
    match object.get_property(QUEUE_ID_PROPERTY) {
        Some(Value::Int(id)) if id > 0 => Ok(id),
        _ => Err(BuiltinError::new(
            "E_PHP_RUNTIME_SYSVMSG_INVALID",
            format!("{name}(): SysvMessageQueue object is no longer valid"),
        )),
    }
}

fn queue_object(id: i64) -> ObjectRef {
    let object = ObjectRef::new_with_display_name(&runtime_class(QUEUE_CLASS), QUEUE_CLASS);
    object.set_property(QUEUE_ID_PROPERTY, Value::Int(id));
    object
}

fn runtime_class(name: &str) -> ClassEntry {
    ClassEntry {
        name: normalize_class_name(name),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: ClassFlags::default(),
    }
}

fn string_key(key: &str) -> ArrayKey {
    ArrayKey::String(PhpString::from_test_str(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OutputBuffer, ReferenceCell};

    #[test]
    fn queue_send_receive_serialized_payload_and_metadata() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        let queue = builtin_msg_get_queue(
            &mut context,
            vec![Value::Int(123), Value::Int(0o600)],
            RuntimeSourceSpan::default(),
        )
        .expect("queue");

        assert_eq!(
            builtin_msg_send(
                &mut context,
                vec![queue.clone(), Value::Int(7), Value::string("payload")],
                RuntimeSourceSpan::default(),
            )
            .expect("send"),
            Value::Bool(true)
        );
        let received_type = ReferenceCell::new(Value::Null);
        let received_message = ReferenceCell::new(Value::Null);
        assert_eq!(
            builtin_msg_receive(
                &mut context,
                vec![
                    queue.clone(),
                    Value::Int(0),
                    Value::Reference(received_type.clone()),
                    Value::Int(1024),
                    Value::Reference(received_message.clone()),
                ],
                RuntimeSourceSpan::default(),
            )
            .expect("receive"),
            Value::Bool(true)
        );
        assert_eq!(received_type.get(), Value::Int(7));
        assert_eq!(received_message.get(), Value::string("payload"));

        let stats = builtin_msg_stat_queue(&mut context, vec![queue], RuntimeSourceSpan::default())
            .expect("stats");
        let Value::Array(stats) = stats else {
            panic!("expected stats array");
        };
        assert_eq!(stats.get(&string_key("msg_qnum")), Some(&Value::Int(0)));
    }
}
