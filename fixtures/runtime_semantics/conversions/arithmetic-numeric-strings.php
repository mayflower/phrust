<?php
// runtime-semantics: category=conversions expect=known_gap known_gap=E_PHP_RUNTIME_NUMERIC_STRING_WARNING_CHANNEL
echo " 42" + 1, "|";
echo "42abc" + 1, "|";
echo "0.5x" + 1, "|";
echo +"0", "|";
echo -"0.0", "\n";
