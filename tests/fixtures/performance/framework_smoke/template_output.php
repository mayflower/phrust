<?php
$rows = array('alpha', 'beta', 'gamma');
echo '<ul>';
foreach ($rows as $row) {
    echo '<li>', strtoupper($row), '</li>';
}
echo '</ul>', "\n";
