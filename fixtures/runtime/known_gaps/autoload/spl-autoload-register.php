<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_AUTOLOAD
spl_autoload_register(function ($class) {
    echo $class;
});

new Phase4AutoloadedClass();
