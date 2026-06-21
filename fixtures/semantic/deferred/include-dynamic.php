<?php
$path = $_GET['path'] ?? 'fallback.php';
$result = include $path;
