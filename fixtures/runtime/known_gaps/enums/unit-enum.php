<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_ENUM_RUNTIME
enum Phase4StatusGap
{
    case Ready;
}

echo Phase4StatusGap::Ready->name;
