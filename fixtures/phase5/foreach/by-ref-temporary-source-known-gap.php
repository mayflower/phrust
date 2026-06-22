<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH
foreach ([1] as &$value) {
    echo $value;
}
