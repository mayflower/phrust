<?php
// phase5-runtime: category=enums expect=pass
enum Priority: string {
    case Low = 'low';
    case High = 'high';
}

echo Priority::from('high')->name . "\n";
echo Priority::tryFrom('missing') === null ? "null\n" : "bad\n";
echo Priority::Low->value . "\n";
