<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_STDLIB_GAP
// PHP reference: var_export() emits reconstructable code for objects.
class ExportBoxFixture
{
    public $value = 3;
}
echo var_export(new ExportBoxFixture(), true), "\n";
