<?php
// runtime-semantics: category=reflection expect=pass
#[Attribute]
class InstantiableAttribute {
}

#[InstantiableAttribute]
class AttributeNewInstanceTarget {
}

$attribute = (new ReflectionClass(AttributeNewInstanceTarget::class))->getAttributes()[0];
$attribute->newInstance();
