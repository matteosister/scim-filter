# SCIM Filter

This library is an implementation of the [SCIM filter specification](https://datatracker.ietf.org/doc/html/rfc7644#section-3.4.2.2).

It exposes a simple api which is the function `scim_filter`

This function takes two arguments:
- the filter string
- a collection of things that implements the Serialize trait from serde.

By applying the filter it will return a Result with either the filtered collection or an error.

Errors can involve filter parsing errors and serialization errors.
