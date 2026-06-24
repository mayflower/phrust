<?php
// runtime-semantics: category=globals expect=pass args=alpha,beta
echo $argc, "\n";
echo $argv[1], ":", $argv[2], "\n";
echo $_SERVER["argc"], "\n";
echo $_SERVER["argv"][2], "\n";
echo empty($_GET), ":", empty($_POST), ":", empty($_COOKIE), ":", empty($_FILES), ":", empty($_REQUEST), "\n";
