--TEST--
redis deterministic fake backend core commands
--SKIPIF--
<?php if (!extension_loaded("redis")) die("skip redis extension not loaded"); ?>
--FILE--
<?php
var_dump(extension_loaded("redis"));
var_dump(class_exists("Redis", false));
$redis = new Redis();
var_dump($redis instanceof Redis);
var_dump(method_exists($redis, "getMultiple"));
var_dump($redis->connect("127.0.0.1"));
var_dump($redis->auth("pw"));
var_dump($redis->select(1));
var_dump($redis->set("alpha", "one"));
var_dump($redis->get("alpha"));
var_dump($redis->setnx("alpha", "two"));
var_dump($redis->incr("count"));
var_dump($redis->incrBy("count", 4));
var_dump($redis->mset(["beta" => "two", "gamma" => "three"]));
$many = $redis->mget(["alpha", "beta", "missing"]);
var_dump($many[0], $many[1], $many[2]);
var_dump($redis->hSet("hash", "field", "value"));
var_dump($redis->hGet("hash", "field"));
var_dump($redis->lPush("list", "left", "middle"));
var_dump($redis->rPop("list"));
var_dump($redis->sAdd("set", "a", "a", "b"));
var_dump($redis->sIsMember("set", "b"));
var_dump($redis->zAdd("zset", 1, "member"));
var_dump($redis->zRange("zset", 0, -1)[0]);
var_dump($redis->ttl("alpha"));
var_dump($redis->del("alpha", "missing"));
var_dump($redis->exists("alpha", "beta"));
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
string(3) "one"
bool(false)
int(1)
int(5)
bool(true)
string(3) "one"
string(3) "two"
bool(false)
int(1)
string(5) "value"
int(2)
string(4) "left"
int(2)
bool(true)
int(1)
string(6) "member"
int(-1)
int(1)
int(1)
