--TEST--
ldap facade surface, options, empty results, errors, and escaping
--SKIPIF--
<?php if (!extension_loaded("ldap")) die("skip ldap extension not loaded"); ?>
--FILE--
<?php
$functions = [
    "ldap_add",
    "ldap_bind",
    "ldap_close",
    "ldap_connect",
    "ldap_count_entries",
    "ldap_delete",
    "ldap_err2str",
    "ldap_errno",
    "ldap_error",
    "ldap_escape",
    "ldap_get_entries",
    "ldap_get_option",
    "ldap_list",
    "ldap_modify",
    "ldap_read",
    "ldap_search",
    "ldap_set_option",
    "ldap_start_tls",
    "ldap_unbind",
];
foreach ($functions as $function) {
    echo $function, ":", (function_exists($function) ? "yes" : "no"), "\n";
}
var_dump(class_exists(LDAP\Connection::class));
var_dump(class_exists(LDAP\Result::class));
var_dump(class_exists(LDAP\ResultEntry::class));
var_dump([
    LDAP_DEREF_NEVER,
    LDAP_DEREF_SEARCHING,
    LDAP_DEREF_FINDING,
    LDAP_DEREF_ALWAYS,
    LDAP_MODIFY_BATCH_ADD,
    LDAP_MODIFY_BATCH_REMOVE,
    LDAP_MODIFY_BATCH_REMOVE_ALL,
    LDAP_MODIFY_BATCH_REPLACE,
    LDAP_MODIFY_BATCH_ATTRIB,
    LDAP_MODIFY_BATCH_MODTYPE,
    LDAP_MODIFY_BATCH_VALUES,
    LDAP_ESCAPE_FILTER,
    LDAP_ESCAPE_DN,
]);
$ldap = ldap_connect("ldap://127.0.0.1", 3389);
var_dump($ldap instanceof LDAP\Connection);
var_dump(ldap_set_option($ldap, LDAP_OPT_PROTOCOL_VERSION, 3));
$option = null;
var_dump(ldap_get_option($ldap, LDAP_OPT_PROTOCOL_VERSION, $option));
var_dump($option);
$result = ldap_search($ldap, "dc=example,dc=org", "(uid=missing)");
var_dump($result instanceof LDAP\Result);
var_dump(ldap_count_entries($ldap, $result));
var_dump(ldap_get_entries($ldap, $result));
var_dump(ldap_first_entry($ldap, $result));
var_dump(ldap_bind($ldap));
var_dump(ldap_errno($ldap));
var_dump(ldap_error($ldap));
var_dump(ldap_err2str(81));
var_dump(ldap_start_tls($ldap));
var_dump(ldap_errno($ldap));
var_dump(ldap_escape("a*(b)\\c", "", LDAP_ESCAPE_FILTER));
var_dump(ldap_escape(" cn=admin ", "", LDAP_ESCAPE_DN));
var_dump(ldap_explode_dn("cn=admin,dc=example,dc=org", 0));
var_dump(ldap_dn2ufn("cn=admin,dc=example,dc=org"));
var_dump(ldap_unbind($ldap));
?>
--EXPECT--
ldap_add:yes
ldap_bind:yes
ldap_close:yes
ldap_connect:yes
ldap_count_entries:yes
ldap_delete:yes
ldap_err2str:yes
ldap_errno:yes
ldap_error:yes
ldap_escape:yes
ldap_get_entries:yes
ldap_get_option:yes
ldap_list:yes
ldap_modify:yes
ldap_read:yes
ldap_search:yes
ldap_set_option:yes
ldap_start_tls:yes
ldap_unbind:yes
bool(true)
bool(true)
bool(true)
array(13) {
  [0]=>
  int(0)
  [1]=>
  int(1)
  [2]=>
  int(2)
  [3]=>
  int(3)
  [4]=>
  int(1)
  [5]=>
  int(2)
  [6]=>
  int(18)
  [7]=>
  int(3)
  [8]=>
  string(6) "attrib"
  [9]=>
  string(7) "modtype"
  [10]=>
  string(6) "values"
  [11]=>
  int(1)
  [12]=>
  int(2)
}
bool(true)
bool(true)
bool(true)
int(3)
bool(true)
int(0)
array(1) {
  ["count"]=>
  int(0)
}
bool(false)
bool(false)
int(81)
string(34) "ldap_bind requires an LDAP backend"
string(25) "Can't contact LDAP server"
bool(false)
int(81)
string(15) "a\2a\28b\29\5cc"
string(16) "\20cn\3dadmin\20"
array(4) {
  [0]=>
  string(5) "admin"
  [1]=>
  string(7) "example"
  [2]=>
  string(3) "org"
  ["count"]=>
  int(3)
}
string(19) "admin, example, org"
bool(true)
