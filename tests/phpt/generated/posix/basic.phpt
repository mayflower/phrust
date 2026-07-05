--TEST--
posix bounded host-backed helpers
--SKIPIF--
<?php if (!extension_loaded("posix")) die("skip posix extension not loaded"); ?>
--FILE--
<?php
var_dump(function_exists("posix_getpid"));
var_dump(is_int(posix_getpid()));
var_dump(posix_getpid() > 0);
var_dump(is_int(posix_getuid()));
var_dump(is_int(posix_getgid()));

$uname = posix_uname();
var_dump(is_array($uname));
var_dump(isset($uname["sysname"], $uname["nodename"], $uname["release"], $uname["version"], $uname["machine"]));

var_dump(posix_access(__FILE__, POSIX_R_OK));
$missing = __DIR__ . "/missing-" . posix_getpid();
var_dump(posix_access($missing, POSIX_F_OK));
var_dump(posix_get_last_error() > 0);
var_dump(is_string(posix_strerror(posix_get_last_error())));

var_dump(is_int(posix_sysconf(POSIX_SC_OPEN_MAX)) || posix_sysconf(POSIX_SC_OPEN_MAX) === false);
var_dump(is_int(posix_pathconf(__DIR__, POSIX_PC_NAME_MAX)) || posix_pathconf(__DIR__, POSIX_PC_NAME_MAX) === false);
var_dump(is_array(posix_times()));

$pw = posix_getpwuid(posix_getuid());
var_dump(is_array($pw));
var_dump(isset($pw["name"], $pw["uid"], $pw["gid"], $pw["dir"], $pw["shell"]));
var_dump(is_array(posix_getpwnam($pw["name"])));

$grp = posix_getgrgid(posix_getgid());
var_dump(is_array($grp));
var_dump(isset($grp["name"], $grp["gid"], $grp["members"]));
var_dump(is_array(posix_getgrnam($grp["name"])));

var_dump(is_array(posix_getgroups()));
$login = posix_getlogin();
var_dump($login === false || is_string($login));
var_dump(posix_kill(posix_getpid(), 0));

$limits = posix_getrlimit();
var_dump(is_array($limits));
var_dump(count($limits) > 0);
$coreLimit = posix_getrlimit(POSIX_RLIMIT_CORE);
var_dump(is_array($coreLimit) && count($coreLimit) === 2);

$isatty = posix_isatty(0);
var_dump($isatty === false || $isatty === true);
$ttyname = posix_ttyname(0);
var_dump($ttyname === false || is_string($ttyname));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
