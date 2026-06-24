<?php
// runtime-semantics: category=reflection expect=pass
interface ReflectionInterfaceMetadata
{
    public function execute(string $value): string;
}

$class = new ReflectionClass(ReflectionInterfaceMetadata::class);
echo $class->getName(), "\n";
echo $class->isInterface() ? "interface" : "class", "\n";
echo $class->isInstantiable() ? "instantiable" : "not-instantiable", "\n";
echo $class->getMethods()[0]->getName(), "\n";
