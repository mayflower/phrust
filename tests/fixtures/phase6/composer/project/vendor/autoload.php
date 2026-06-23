<?php
spl_autoload_register('phase6_project_autoload');

function phase6_project_autoload($class)
{
    $prefix = 'Phase6\\ComposerProject\\';
    $prefix_len = strlen($prefix);
    if (strncmp($class, $prefix, $prefix_len) !== 0) {
        return;
    }

    $relative = substr($class, $prefix_len);
    $path = str_replace('\\', '/', $relative) . '.php';
    $resolved = stream_resolve_include_path($path);
    if ($resolved !== false) {
        include $resolved;
    }
}
