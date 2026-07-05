--TEST--
imap facade surface, empty mailbox, status, and backend errors
--SKIPIF--
<?php if (!extension_loaded("imap")) die("skip imap extension not loaded"); ?>
--FILE--
<?php
$functions = [
    "imap_open",
    "imap_close",
    "imap_headers",
    "imap_fetchbody",
    "imap_fetchstructure",
    "imap_search",
    "imap_append",
    "imap_delete",
    "imap_expunge",
    "imap_status",
    "imap_errors",
    "imap_alerts",
    "imap_last_error",
];
foreach ($functions as $function) {
    echo $function, ":", (function_exists($function) ? "yes" : "no"), "\n";
}
var_dump(class_exists(IMAP\Connection::class));
var_dump([
    OP_READONLY,
    OP_HALFOPEN,
    CL_EXPUNGE,
    FT_UID,
    FT_PEEK,
    SA_MESSAGES,
    SA_RECENT,
    SA_UNSEEN,
    SA_UIDNEXT,
    SA_UIDVALIDITY,
    SA_ALL,
]);
$imap = imap_open("{127.0.0.1:143/imap}INBOX", "user", "secret", OP_HALFOPEN);
var_dump($imap instanceof IMAP\Connection);
var_dump(imap_ping($imap));
var_dump(imap_num_msg($imap));
var_dump(imap_num_recent($imap));
var_dump(imap_headers($imap));
var_dump(imap_search($imap, "ALL"));
var_dump(imap_fetchbody($imap, 1, "1", FT_PEEK));
var_dump(imap_fetchstructure($imap, 1));
var_dump(imap_fetchheader($imap, 1));
var_dump(imap_headerinfo($imap, 1));
$check = imap_check($imap);
var_dump($check->Driver, $check->Mailbox, $check->Nmsgs, $check->Recent);
$status = imap_status($imap, "{127.0.0.1:143/imap}INBOX", SA_ALL);
var_dump($status->messages, $status->recent, $status->unseen, $status->uidnext, $status->uidvalidity);
$info = imap_mailboxmsginfo($imap);
var_dump($info->Nmsgs, $info->Recent, $info->Unread, $info->Deleted, $info->Size);
var_dump(imap_delete($imap, 1));
var_dump(imap_expunge($imap));
var_dump(imap_append($imap, "{127.0.0.1:143/imap}INBOX", "Subject: Test\r\n\r\nBody"));
var_dump(imap_last_error());
var_dump(imap_errors());
var_dump(imap_errors());
var_dump(imap_alerts());
var_dump(imap_close($imap, CL_EXPUNGE));
var_dump(imap_ping($imap));
?>
--EXPECTF--
imap_open:yes
imap_close:yes
imap_headers:yes
imap_fetchbody:yes
imap_fetchstructure:yes
imap_search:yes
imap_append:yes
imap_delete:yes
imap_expunge:yes
imap_status:yes
imap_errors:yes
imap_alerts:yes
imap_last_error:yes
bool(true)
array(11) {
  [0]=>
  int(2)
  [1]=>
  int(64)
  [2]=>
  int(32768)
  [3]=>
  int(1)
  [4]=>
  int(2)
  [5]=>
  int(1)
  [6]=>
  int(2)
  [7]=>
  int(4)
  [8]=>
  int(8)
  [9]=>
  int(16)
  [10]=>
  int(31)
}
bool(true)
bool(true)
int(0)
int(0)
array(0) {
}
bool(false)
string(0) ""
object(stdClass)#%d (1) {
  ["type"]=>
  int(0)
}
string(0) ""
object(stdClass)#%d (4) {
  ["subject"]=>
  string(0) ""
  ["fromaddress"]=>
  string(0) ""
  ["date"]=>
  string(0) ""
  ["Msgno"]=>
  int(1)
}
string(11) "phrust-imap"
string(25) "{127.0.0.1:143/imap}INBOX"
int(0)
int(0)
int(0)
int(0)
int(0)
int(1)
int(1)
int(0)
int(0)
int(0)
int(0)
int(0)
bool(true)
bool(true)
bool(false)
string(30) "IMAP backend is not configured"
array(1) {
  [0]=>
  string(30) "IMAP backend is not configured"
}
bool(false)
bool(false)
bool(true)
bool(false)
