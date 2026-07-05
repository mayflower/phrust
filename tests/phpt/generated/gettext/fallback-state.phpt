--TEST--
gettext fallback translations and request-local binding state
--SKIPIF--
<?php if (!extension_loaded("gettext")) die("skip gettext extension not loaded"); ?>
--FILE--
<?php
var_dump(extension_loaded("gettext"));
var_dump(function_exists("gettext"), function_exists("_"));
var_dump(textdomain(null));
var_dump(textdomain("phrust"));
var_dump(gettext("Hello"));
var_dump(_("Alias"));
var_dump(dgettext("phrust", "Domain message"));
var_dump(dcgettext("phrust", "Categorized", LC_MESSAGES));
var_dump(ngettext("one", "many", 1));
var_dump(ngettext("one", "many", 2));
var_dump(dngettext("phrust", "item", "items", 3));
var_dump(dcngettext("phrust", "entry", "entries", 1, LC_MESSAGES));
$bound = bindtextdomain("phrust", __DIR__);
var_dump(is_string($bound));
var_dump(bindtextdomain("phrust", null) === $bound);
var_dump(bind_textdomain_codeset("phrust", null));
var_dump(bind_textdomain_codeset("phrust", "UTF-8"));
var_dump(bind_textdomain_codeset("phrust", null));
try {
    dcgettext("phrust", "bad", LC_ALL);
} catch (ValueError $e) {
    echo "value-error\n";
}
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
string(8) "messages"
string(6) "phrust"
string(5) "Hello"
string(5) "Alias"
string(14) "Domain message"
string(11) "Categorized"
string(3) "one"
string(4) "many"
string(5) "items"
string(5) "entry"
bool(true)
bool(true)
bool(false)
string(5) "UTF-8"
string(5) "UTF-8"
value-error
