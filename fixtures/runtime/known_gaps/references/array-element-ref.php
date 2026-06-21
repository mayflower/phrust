<?php
$array = [1];
$alias =& $array[0];
$alias = 2;
echo $array[0];
