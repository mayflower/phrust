--TEST--
sysvsem deterministic semaphore compatibility slice
--EXTENSIONS--
sysvsem
--FILE--
<?php
echo extension_loaded('sysvsem') ? "loaded\n" : "missing\n";
echo function_exists('sem_get') ? "function\n" : "no function\n";
echo class_exists('SysvSemaphore') ? "class\n" : "no class\n";

$sem = sem_get(0x53454d31, 1, 0600);
var_dump($sem instanceof SysvSemaphore);
var_dump(sem_acquire($sem));
var_dump(sem_acquire($sem, true));
var_dump(sem_release($sem));
var_dump(sem_acquire($sem, true));
var_dump(sem_release($sem));
var_dump(sem_remove($sem));
var_dump(sem_acquire($sem, true));
?>
--EXPECT--
loaded
function
class
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
bool(true)
bool(true)
bool(false)
