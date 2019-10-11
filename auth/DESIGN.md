## Overview

Using OpenID via Google:

https://developers.google.com/identity/protocols/OpenIDConnect#discovery

Build a service which can:

1. Validate that a user owns a particular google/openid account
2. Exchange that validation for a token
3. Verify that a token corresponds to a given user

Service definition:

```
Login{} -> LoginChallenge{ url: String, token: String }
```

If the login challenge succeeds, the token will become valid for accessing that
user's account.

```
Authenticate{token: String} -> AuthenticationResult{ success: bool, userid: String }
```

Later, services can send Authenticate RPC to check whether the token is valid
and which user it represents.
