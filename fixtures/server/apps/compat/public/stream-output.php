<?php
echo "prefix|";
$stdout = fopen("php://stdout", "wb");
fwrite($stdout, "stdout|");
$memory = fopen("php://memory", "w+");
fwrite($memory, "memory");
rewind($memory);
echo stream_get_contents($memory), "\n";
