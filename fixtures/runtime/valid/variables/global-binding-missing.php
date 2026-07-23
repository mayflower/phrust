<?php

class MissingGlobalBindingTarget
{
    public bool $enabled = true;

    public function run(): void
    {
        global $native_missing_method_global;
        if (false !== $this->enabled && $native_missing_method_global) {
            echo "unexpected\n";
        }
    }
}

function bind_missing_function_global(string $path): void
{
    global $native_missing_function_global;
    if (!file_exists($path)) {
        return;
    }
    var_dump(is_array($native_missing_function_global));
}

var_dump(isset($GLOBALS['native_missing_method_global']));
var_dump(isset($GLOBALS['native_missing_function_global']));
(new MissingGlobalBindingTarget())->run();
bind_missing_function_global(__FILE__);
