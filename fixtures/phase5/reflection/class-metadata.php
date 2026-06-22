<?php
// phase5-runtime: category=reflection expect=pass
interface ReflectionMetadataInterface
{
    public function contract(string $value): string;
}

final class ReflectionMetadataTarget implements ReflectionMetadataInterface
{
    public const LABEL = "meta";
    public int $id = 4;

    public function contract(string $value = "fallback"): string
    {
        return $value;
    }
}

$class = new ReflectionClass(ReflectionMetadataTarget::class);
echo $class->getName(), "\n";
echo $class->isFinal() ? "final" : "not-final", "\n";
echo $class->isInterface() ? "interface" : "class", "\n";
echo $class->isInstantiable() ? "instantiable" : "not-instantiable", "\n";
echo $class->getInterfaceNames()[0], "\n";
echo $class->getMethods()[0]->getName(), "\n";
echo $class->getProperties()[0]->getName(), "\n";
echo $class->getConstants()["LABEL"], "\n";
echo $class->getConstant("LABEL"), "\n";
