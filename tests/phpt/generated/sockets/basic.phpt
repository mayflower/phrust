--TEST--
sockets loopback TCP basics
--SKIPIF--
<?php if (!extension_loaded("sockets")) die("skip sockets extension not loaded"); ?>
--FILE--
<?php
$server = socket_create(AF_INET, SOCK_STREAM, SOL_TCP);
echo get_class($server), "\n";
var_dump(socket_bind($server, "127.0.0.1", 0));
var_dump(socket_listen($server, 1));
$addr = null;
$port = null;
var_dump(socket_getsockname($server, $addr, $port));
echo $addr, "\n";
echo is_int($port) && $port > 0 ? "port\n" : "no-port\n";

$client = socket_create(AF_INET, SOCK_STREAM, SOL_TCP);
var_dump(socket_connect($client, "127.0.0.1", $port));
$accepted = socket_accept($server);
echo get_class($accepted), "\n";

var_dump(socket_write($client, "ping"));
echo socket_read($accepted, 4, PHP_BINARY_READ), "\n";
var_dump(socket_write($accepted, "pong"));
echo socket_read($client, 4, PHP_BINARY_READ), "\n";

socket_close($accepted);
socket_close($client);
socket_close($server);
var_dump(socket_create(999999, 999999, 999999));
echo is_string(socket_strerror(socket_last_error())) ? "error-string\n" : "not-string\n";
socket_clear_error();
var_dump(socket_last_error());
?>
--EXPECT--
Socket
bool(true)
bool(true)
bool(true)
127.0.0.1
port
bool(true)
Socket
int(4)
ping
int(4)
pong
bool(false)
error-string
int(0)
