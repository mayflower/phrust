<?php
// phase5-runtime: category=reflection expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_ATTRIBUTE_NEWINSTANCE
#[Attribute]
class InstantiableAttribute {
}

#[InstantiableAttribute]
class AttributeNewInstanceTarget {
}

$attribute = (new ReflectionClass(AttributeNewInstanceTarget::class))->getAttributes()[0];
$attribute->newInstance();
