--TEST--
pdo_pgsql: live query and prepared statement through DSN gate
--DESCRIPTION--
Generated opt-in live PostgreSQL PDO query contract.
--SKIPIF--
<?php
if (!extension_loaded("pdo") || !extension_loaded("pdo_pgsql")) {
    die("skip pdo_pgsql extension is not loaded");
}
if (getenv("PHRUST_POSTGRES_TEST_DSN") === false || getenv("PHRUST_POSTGRES_TEST_DSN") === "") {
    die("skip PHRUST_POSTGRES_TEST_DSN is not configured");
}
?>
--FILE--
<?php
$parts = parse_url(getenv("PHRUST_POSTGRES_TEST_DSN"));
$host = $parts["host"] ?? "127.0.0.1";
$user = isset($parts["user"]) ? rawurldecode($parts["user"]) : "";
$pass = isset($parts["pass"]) ? rawurldecode($parts["pass"]) : "";
$dbName = isset($parts["path"]) ? ltrim($parts["path"], "/") : "";
$port = $parts["port"] ?? null;

$dsn = "pgsql:host=" . $host;
if ($port !== null) {
    $dsn .= ";port=" . $port;
}
if ($dbName !== "") {
    $dsn .= ";dbname=" . $dbName;
}
$dsn .= ";sslmode=disable";

$pdo = new PDO($dsn, $user, $pass);
var_dump($pdo->getAttribute(PDO::ATTR_DRIVER_NAME));

$result = $pdo->query("SELECT 1 AS one");
var_dump($result instanceof PDOStatement);
var_dump($result->fetch(PDO::FETCH_ASSOC));

$statement = $pdo->prepare("SELECT ?::bigint AS two");
var_dump($statement->execute([2]));
var_dump($statement->fetchColumn());
var_dump($pdo->quote("a'b"));
?>
--EXPECT--
string(5) "pgsql"
bool(true)
array(1) {
  ["one"]=>
  int(1)
}
bool(true)
int(2)
string(6) "'a''b'"
