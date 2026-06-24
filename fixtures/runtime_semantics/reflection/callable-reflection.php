<?php
// runtime-semantics: category=reflection expect=pass
function reflection_callable_fixture(int $value): int {
    return $value;
}

$callable = reflection_callable_fixture(...);
$function = new ReflectionFunction($callable);
$parameters = $function->getParameters();

echo $function->getName(), "\n";
echo $function->getReturnType()->getName(), "\n";
echo $parameters[0]->getName(), ":", $parameters[0]->getType()->getName(), "\n";
