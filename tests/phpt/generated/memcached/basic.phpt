--TEST--
memcached deterministic fake backend core cache surface
--SKIPIF--
<?php if (!extension_loaded("memcached")) die("skip memcached extension not loaded"); ?>
--FILE--
<?php
var_dump(extension_loaded("memcached"));
var_dump(class_exists("Memcached", false));
$m = new Memcached();
var_dump($m instanceof Memcached);
var_dump(method_exists($m, "getMulti"));
var_dump(Memcached::RES_SUCCESS);
var_dump(Memcached::RES_NOTFOUND);
var_dump($m->addServer("127.0.0.1", 11211));
var_dump($m->set("alpha", "one"));
var_dump($m->get("alpha"));
var_dump($m->add("alpha", "two"));
var_dump($m->getResultCode());
var_dump($m->replace("alpha", "three"));
var_dump($m->get("alpha"));
var_dump($m->setMulti(["beta" => "two", "gamma" => "three"]));
$many = $m->getMulti(["alpha", "beta", "missing"]);
var_dump($many["alpha"], $many["beta"], isset($many["missing"]));
var_dump($m->increment("count", 2, 10));
var_dump($m->decrement("count", 3));
var_dump($m->append("alpha", "!"));
var_dump($m->get("alpha"));
var_dump($m->touch("alpha", 60));
var_dump($m->delete("alpha"));
var_dump($m->get("alpha"));
var_dump($m->getResultCode());
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
int(0)
int(16)
bool(true)
bool(true)
string(3) "one"
bool(false)
int(16)
bool(true)
string(5) "three"
bool(true)
string(5) "three"
string(3) "two"
bool(false)
int(10)
int(7)
bool(true)
string(6) "three!"
bool(true)
bool(true)
bool(false)
int(16)
