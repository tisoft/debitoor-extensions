# debitoor-extensions

Extensions for the Debitoor accounting service.

## Running locally
You need to register an application in Debitoor: https://app.debitoor.com/account/apps/new
Callback URL must be http://localhost:8080 all other fileds can be chosen freely. Save the app and note the Client ID and Secret.


Install rust https://www.rust-lang.org/en-US/install.html


Check out this project and set it to use the rust nighlty compiler.

    rustup override set nightly

You can now run the following in a shell:

    CLIENT_ID='the client id from debitoor' CLIENT_SECRET='the client secret from debitoor' cargo run

You can open your browser now to http://localhost:8080 which will redirect you to debitoor for authentication.

## Build Status

[![Build Status](https://travis-ci.org/tisoft/debitoor-extensions.svg?branch=master)](https://travis-ci.org/tisoft/debitoor-extensions)