<?php
// phase4: kind=valid expected_stdout="ok0|ok0\n"
$x = 0;
$y = 0;
echo ($x && ++$y) ? "bad" : "ok";
echo $y;
echo "|";
echo (true || ++$y) ? "ok" : "bad";
echo $y;
echo "\n";
