<?php
// runtime-fixture: corpus=pass
$config = [
    "env" => "prod",
    "database" => [
        "host" => "db",
        "replicas" => [1, 2],
    ],
];

echo $config["env"], "|", $config["database"]["host"], "|", $config["database"]["replicas"][0], "\n";
