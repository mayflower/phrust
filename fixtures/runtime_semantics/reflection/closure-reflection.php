<?php
// runtime-semantics: category=reflection expect=pass
$captured = "cap";
$closure = function (int $value) use ($captured): string {
    return $captured . $value;
};

$function = new ReflectionFunction($closure);
$parameters = $function->getParameters();
$static = $function->getStaticVariables();

echo $function->isClosure() ? "closure" : "function", "\n";
echo $function->getReturnType()->getName(), "\n";
echo $parameters[0]->getName(), ":", $parameters[0]->getType()->getName(), "\n";
echo $static["captured"], "\n";
echo $function->getClosureScopeClass() === null ? "no-scope" : "scope", "\n";
