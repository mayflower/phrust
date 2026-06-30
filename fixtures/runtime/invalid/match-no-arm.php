<?php
// runtime-fixture: kind=invalid diagnostic_id=E_PHP_VM_UNCAUGHT_EXCEPTION
echo match (2) {
    0 => "zero",
    1 => "one",
};
