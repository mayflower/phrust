--TEST--
FFI INI policy is visible and read-only by default
--EXTENSIONS--
ffi
--FILE--
<?php
var_dump(ini_get('ffi.enable'));
var_dump(ini_get('ffi.preload'));
$flat = ini_get_all('ffi', false);
foreach (['ffi.enable', 'ffi.preload'] as $name) {
    echo $name, '=', $flat[$name], "\n";
}
var_dump(isset($flat['default_charset']));
var_dump(ini_set('ffi.enable', '1'));
var_dump(ini_get('ffi.enable'));
$details = ini_get_all('ffi');
echo $details['ffi.enable']['global_value'], '|',
    $details['ffi.enable']['local_value'], '|',
    $details['ffi.enable']['access'], "\n";
?>
--EXPECT--
string(7) "preload"
string(0) ""
ffi.enable=preload
ffi.preload=
bool(false)
bool(false)
string(7) "preload"
preload|preload|4
