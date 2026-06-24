<?php
// stdlib-diff: id=STDLIB_REFLECTION_FUNCTION area=spl-reflection expect=pass
$fn = new ReflectionFunction("count");
echo $fn->getName(), "\n";
echo $fn->isInternal() ? "internal\n" : "user\n";
echo $fn->getFileName() === false ? "nofile\n" : "file\n";
echo $fn->getNumberOfParameters(), "|", $fn->getNumberOfRequiredParameters(), "\n";
$params = $fn->getParameters();
echo $params[0]->getName(), "\n";
