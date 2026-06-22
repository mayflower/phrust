<?php
// phase5-runtime: category=reflection expect=pass
function reflected_prompt27_function(int $count, string $label = "ok"): bool
{
    return true;
}

$function = new ReflectionFunction("reflected_prompt27_function");
$returnType = $function->getReturnType();
$parameters = $function->getParameters();

echo $function->getName(), "\n";
echo $returnType->getName(), ":", ($returnType->isBuiltin() ? "builtin" : "class"), "\n";
echo $function->getNumberOfParameters(), ":", $function->getNumberOfRequiredParameters(), "\n";
echo $parameters[0]->getName(), ":", $parameters[0]->getType()->getName(), ":", ($parameters[0]->isOptional() ? "optional" : "required"), "\n";
echo $parameters[1]->getName(), ":", $parameters[1]->getType()->getName(), ":", $parameters[1]->getDefaultValue(), "\n";
