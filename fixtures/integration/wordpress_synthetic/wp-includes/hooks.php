<?php
$synthetic_hooks = [];

function add_action($hook, $callback) {
    global $synthetic_hooks;
    if (!isset($synthetic_hooks[$hook])) {
        $synthetic_hooks[$hook] = [];
    }
    $synthetic_hooks[$hook][count($synthetic_hooks[$hook])] = $callback;
}

function do_action($hook) {
    global $synthetic_hooks;
    foreach ($synthetic_hooks[$hook] ?? [] as $callback) {
        $callback();
    }
}
