<?php
$message = "parent";
eval('$message = $message . "|eval";');
echo $message, "\n";
