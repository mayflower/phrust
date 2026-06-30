<?php
namespace RuntimeNamespaceFixture;

use function RuntimeNamespaceFixture\included_function as imported_function;
use const RuntimeNamespaceFixture\INCLUDED_CONST as IMPORTED_CONST;

include __DIR__ . "/_data/contribution.php";

echo included_function(), "|";
echo imported_function(), "|";
echo INCLUDED_CONST, "|";
echo IMPORTED_CONST, "|";
echo (new IncludedClass())->value(), "|";
echo strlen("abc"), "\n";
