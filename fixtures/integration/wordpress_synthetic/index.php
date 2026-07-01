<?php
require __DIR__ . '/wp-load.php';

ob_start();
do_action('init');
$body = ob_get_clean();

echo "front:" . ($_SERVER['REQUEST_URI'] ?? 'missing') . "\n";
echo $body;
