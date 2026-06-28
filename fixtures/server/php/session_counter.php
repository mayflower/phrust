<?php
session_start();

$n = ($_SESSION["n"] ?? 0) + 1;
$_SESSION["n"] = $n;
echo "id=", session_id(), "\n";
echo "n=", $n, "\n";
echo "status=", session_status(), "\n";
