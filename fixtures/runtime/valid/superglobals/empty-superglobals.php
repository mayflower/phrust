<?php
echo isset($_GET['x']) ? 'get-set' : 'get-empty';
echo "|";
echo isset($_POST['x']) ? 'post-set' : 'post-empty';
echo "|";
echo isset($_REQUEST['x']) ? 'request-set' : 'request-empty';
echo "|";
echo isset($_ENV['HOST_SECRET']) ? 'env-leaked' : 'env-empty';
echo "\n";
