<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_VM_UNRESOLVED_CALLABLE
namespace Demo\Calls;

function suffix($value) {
    return $value . "N";
}

$callable = __NAMESPACE__ . "\\suffix";
echo $callable("A");
