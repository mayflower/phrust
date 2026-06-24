<?php
// runtime-semantics: category=reflection expect=pass
class ReflectionMemberMetadata
{
    protected const CODE = 42;
    private string $name = "initial";

    public static function run(int $count = 7): string
    {
        return "run";
    }
}

$class = new ReflectionClass(ReflectionMemberMetadata::class);
$method = $class->getMethod("run");
$property = $class->getProperty("name");
$constant = $class->getReflectionConstant("CODE");
$parameter = $method->getParameters()[0];

echo $method->getDeclaringClass()->getName(), ":", $method->getName(), ":", ($method->isStatic() ? "static" : "instance"), ":", $method->getReturnType()->getName(), "\n";
echo $parameter->getName(), ":", $parameter->getType()->getName(), ":", $parameter->getDefaultValue(), "\n";
echo $property->getName(), ":", ($property->isPrivate() ? "private" : "not-private"), ":", $property->getType()->getName(), ":", $property->getDefaultValue(), "\n";
echo $constant->getName(), ":", ($constant->isProtected() ? "protected" : "not-protected"), ":", $constant->getValue(), "\n";
