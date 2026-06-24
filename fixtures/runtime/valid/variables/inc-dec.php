<?php
// runtime-fixture: kind=valid expected_stdout="1|3|3|1\n"
$a = 1;
echo $a++, "|", ++$a, "|", $a--, "|", --$a, "\n";
