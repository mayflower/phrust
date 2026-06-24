<?php
// stdlib-diff: id=STDLIB_TYPE_VALUE_ERRORS area=stdlib expect=pass
try {
    strlen([]);
} catch (TypeError $e) {
    echo "type\n";
}
try {
    explode('', 'abc');
} catch (ValueError $e) {
    echo "value\n";
}
