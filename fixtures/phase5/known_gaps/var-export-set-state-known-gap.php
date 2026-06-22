<?php
// phase5-runtime: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_PHASE6_GAP
// PHP reference: var_export() emits reconstructable code for objects.
class Prompt43ExportBox
{
    public $value = 3;
}
echo var_export(new Prompt43ExportBox(), true), "\n";
