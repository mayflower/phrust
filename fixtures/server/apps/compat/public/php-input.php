<?php
$body = file_get_contents("php://input");
echo "len=", strlen($body), "\n";
echo "body=", $body, "\n";
echo "post-count=", count($_POST), "\n";
