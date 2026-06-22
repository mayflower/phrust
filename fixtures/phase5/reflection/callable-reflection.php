<?php
// phase5-runtime: category=reflection expect=pass
function prompt28_callable(int $value): int {
    return $value;
}

$callable = prompt28_callable(...);
$function = new ReflectionFunction($callable);
$parameters = $function->getParameters();

echo $function->getName(), "\n";
echo $function->getReturnType()->getName(), "\n";
echo $parameters[0]->getName(), ":", $parameters[0]->getType()->getName(), "\n";
