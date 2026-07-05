--TEST--
ssh2 facade surface, handles, and no-backend failures
--SKIPIF--
<?php if (!extension_loaded("ssh2")) die("skip ssh2 extension not loaded"); ?>
--FILE--
<?php
$functions = [
    "ssh2_connect",
    "ssh2_auth_password",
    "ssh2_auth_pubkey_file",
    "ssh2_exec",
    "ssh2_shell",
    "ssh2_scp_send",
    "ssh2_scp_recv",
    "ssh2_sftp",
    "ssh2_sftp_realpath",
    "ssh2_fingerprint",
    "ssh2_methods_negotiated",
    "ssh2_tunnel",
];
foreach ($functions as $function) {
    echo $function, ":", (function_exists($function) ? "yes" : "no"), "\n";
}
var_dump(class_exists(SSH2\Session::class));
var_dump(class_exists(SSH2\Sftp::class));
var_dump([
    SSH2_FINGERPRINT_MD5,
    SSH2_FINGERPRINT_SHA1,
    SSH2_FINGERPRINT_HEX,
    SSH2_FINGERPRINT_RAW,
    SSH2_TERM_UNIT_CHARS,
    SSH2_TERM_UNIT_PIXELS,
    SSH2_DEFAULT_TERMINAL,
    SSH2_DEFAULT_TERM_WIDTH,
    SSH2_DEFAULT_TERM_HEIGHT,
    SSH2_DEFAULT_TERM_UNIT,
]);
$session = ssh2_connect("127.0.0.1", 22);
var_dump($session instanceof SSH2\Session);
var_dump(ssh2_auth_none($session, "user"));
var_dump(ssh2_auth_password($session, "user", "secret"));
var_dump(ssh2_fingerprint($session, SSH2_FINGERPRINT_SHA1 | SSH2_FINGERPRINT_HEX));
$methods = ssh2_methods_negotiated($session);
var_dump($methods["kex"], $methods["hostkey"], $methods["crypt_cs"]);
$sftp = ssh2_sftp($session);
var_dump($sftp instanceof SSH2\Sftp);
var_dump(ssh2_sftp_realpath($sftp, "/tmp/demo"));
var_dump(ssh2_sftp_stat($sftp, "/tmp/demo"));
var_dump(ssh2_exec($session, "id"));
var_dump(ssh2_shell($session));
var_dump(ssh2_scp_send($session, __FILE__, "/tmp/basic.phpt"));
var_dump(ssh2_scp_recv($session, "/tmp/basic.phpt", __DIR__ . "/missing.out"));
var_dump(ssh2_tunnel($session, "127.0.0.1", 80));
var_dump(ssh2_disconnect($session));
var_dump(ssh2_fingerprint($session));
?>
--EXPECT--
ssh2_connect:yes
ssh2_auth_password:yes
ssh2_auth_pubkey_file:yes
ssh2_exec:yes
ssh2_shell:yes
ssh2_scp_send:yes
ssh2_scp_recv:yes
ssh2_sftp:yes
ssh2_sftp_realpath:yes
ssh2_fingerprint:yes
ssh2_methods_negotiated:yes
ssh2_tunnel:yes
bool(true)
bool(true)
array(10) {
  [0]=>
  int(0)
  [1]=>
  int(1)
  [2]=>
  int(0)
  [3]=>
  int(2)
  [4]=>
  int(0)
  [5]=>
  int(1)
  [6]=>
  string(7) "vanilla"
  [7]=>
  int(80)
  [8]=>
  int(25)
  [9]=>
  int(0)
}
bool(true)
array(2) {
  [0]=>
  string(8) "password"
  [1]=>
  string(9) "publickey"
}
bool(false)
string(0) ""
string(0) ""
string(0) ""
string(0) ""
bool(true)
string(9) "/tmp/demo"
bool(false)
bool(false)
bool(false)
bool(false)
bool(false)
bool(false)
bool(true)
bool(false)
