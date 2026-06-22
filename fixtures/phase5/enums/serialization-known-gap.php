<?php
// phase5-runtime: category=enums expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_ENUM_SERIALIZATION
enum SerializableStatus {
    case Ready;
}

echo serialize(SerializableStatus::Ready);
