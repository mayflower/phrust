<?php
// phase5: kind=known_gap id=E_PHP_VM_UNKNOWN_CLASS
spl_autoload_register(function ($class) {
    echo $class;
});

new Phase4AutoloadedClass();
