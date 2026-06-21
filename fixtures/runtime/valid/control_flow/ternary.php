<?php
// phase4: kind=valid expected_stdout="yes|fallback|kept\n"
echo true ? "yes" : "no";
echo "|";
echo false ?: "fallback";
echo "|";
echo "kept" ?: "bad";
echo "\n";
