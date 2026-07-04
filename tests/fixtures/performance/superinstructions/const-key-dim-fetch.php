<?php
// A/B probe for the fused constant-key dimension fetch: constant string
// and int keys read in a loop from record- and packed-shaped arrays.
$config = array("host" => "db.local", "port" => 5432, "name" => "app");
$row = array(10, 20, 30);
$out = "";
$total = 0;
for ($i = 0; $i < 3; $i++) {
    $out = $out . $config["host"] . ":" . $config["port"] . ";";
    $total = $total + $row[1] + $row[2];
}
echo $out, "\n";
echo $total, "\n";
echo $config["missing"] ?? "default", "\n";
// Chained fetches keep a bare constant-key load ahead of the inner
// dimension fetch (the outer fetch is not a fusable producer).
$grid = array(array(1, 2), array(3, 4));
$picked = 0;
for ($i = 0; $i < 3; $i++) {
    $picked = $picked + $grid[1][0] + $grid[0][1];
}
echo $picked, "\n";
