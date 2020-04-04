# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_version**](DefaultApi.md#api_version) | **GET** /system/api-version | Returns API version
[**create_user**](DefaultApi.md#create_user) | **POST** /users | Creates new user
[**delete_run**](DefaultApi.md#delete_run) | **DELETE** /runs/{id} | Deletes run
[**get_contest**](DefaultApi.md#get_contest) | **GET** /contests/{name} | Finds contest by name
[**get_contest_standings**](DefaultApi.md#get_contest_standings) | **GET** /contests/{name}/standings | Returns standings as JSON object
[**get_run**](DefaultApi.md#get_run) | **GET** /runs/{id} | Loads run by id
[**get_run_binary**](DefaultApi.md#get_run_binary) | **GET** /runs/{id}/binary | Returns run build artifact as base64-encoded JSON string
[**get_run_live_status**](DefaultApi.md#get_run_live_status) | **GET** /runs/{id}/live | returns incremental Live Status
[**get_run_protocol**](DefaultApi.md#get_run_protocol) | **GET** /runs/{id}/protocol | Returns invocation protocol as JSON document
[**get_run_source**](DefaultApi.md#get_run_source) | **GET** /runs/{id}/source | Returns run source as base64-encoded JSON string
[**is_dev**](DefaultApi.md#is_dev) | **GET** /system/is-dev | Returns if JJS is running in development mode.
[**list_contest_problems**](DefaultApi.md#list_contest_problems) | **GET** /contests/{name}/problems | Lists all problems in contest `name`
[**list_contests**](DefaultApi.md#list_contests) | **GET** /contests | Lists contests
[**list_runs**](DefaultApi.md#list_runs) | **GET** /runs | List runs
[**list_toolchains**](DefaultApi.md#list_toolchains) | **GET** /toolchains | Lists toolchains
[**log_in**](DefaultApi.md#log_in) | **POST** /auth/simple | Login using login and password
[**patch_run**](DefaultApi.md#patch_run) | **PATCH** /runs/{id} | Modifies run
[**submit_run**](DefaultApi.md#submit_run) | **POST** /runs | Submit run



## api_version

> crate::models::ApiVersion api_version()
Returns API version

Version is returned in format {major: MAJOR, minor: MINOR}. MAJOR component is incremented, when backwards-incompatible changes were made. MINOR component is incremented, when backwards-compatible changes were made.  It means, that if you developed application with apiVersion X.Y, your application should assert that MAJOR = X and MINOR >= Y

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::ApiVersion**](ApiVersion.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_user

> crate::models::User create_user(user_create_params)
Creates new user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_create_params** | [**UserCreateParams**](UserCreateParams.md) |  | [required] |

### Return type

[**crate::models::User**](User.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_run

> delete_run(id)
Deletes run

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |

### Return type

 (empty response body)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_contest

> crate::models::Contest get_contest(name)
Finds contest by name

If contest with this name does not exists, `null` is returned

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | **String** |  | [required] |

### Return type

[**crate::models::Contest**](Contest.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_contest_standings

> serde_json::Value get_contest_standings(name)
Returns standings as JSON object

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_run

> crate::models::Run get_run(id)
Loads run by id

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |

### Return type

[**crate::models::Run**](Run.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_run_binary

> String get_run_binary(id)
Returns run build artifact as base64-encoded JSON string

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |

### Return type

**String**

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_run_live_status

> crate::models::RunLiveStatusUpdate get_run_live_status(id)
returns incremental Live Status

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |

### Return type

[**crate::models::RunLiveStatusUpdate**](RunLiveStatusUpdate.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_run_protocol

> serde_json::Value get_run_protocol(id, compile_log, test_data, output, answer, resource_usage)
Returns invocation protocol as JSON document

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |
**compile_log** | Option<**bool**> | If false, compilation logs will be excluded |  |
**test_data** | Option<**bool**> | If false, test data will be excluded for all tests |  |
**output** | Option<**bool**> | If false, solution stdout&stderr will be excluded for all tests |  |
**answer** | Option<**bool**> | If false, correct answer will be excluded for all tests |  |
**resource_usage** | Option<**bool**> | If false, resource usage will be excluded for all tests |  |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: /application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_run_source

> String get_run_source(id)
Returns run source as base64-encoded JSON string

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |

### Return type

**String**

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## is_dev

> bool is_dev()
Returns if JJS is running in development mode.

Please note that you don't have to respect this information, but following is recommended:  - Display it in each page/view.  - Change theme.  - On login view, add button \"login as root\".

### Parameters

This endpoint does not need any parameter.

### Return type

**bool**

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_contest_problems

> Vec<crate::models::Problem> list_contest_problems(name)
Lists all problems in contest `name`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | **String** |  | [required] |

### Return type

[**Vec<crate::models::Problem>**](Problem.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_contests

> Vec<crate::models::Contest> list_contests()
Lists contests

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<crate::models::Contest>**](Contest.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_runs

> Vec<crate::models::Run> list_runs()
List runs

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<crate::models::Run>**](Run.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_toolchains

> Vec<crate::models::Toolchain> list_toolchains()
Lists toolchains

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<crate::models::Toolchain>**](Toolchain.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## log_in

> crate::models::SessionToken log_in(simple_auth_params)
Login using login and password

In future, other means to authn will be added. See `SessionToken` documentation for more details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**simple_auth_params** | [**SimpleAuthParams**](SimpleAuthParams.md) |  | [required] |

### Return type

[**crate::models::SessionToken**](SessionToken.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## patch_run

> crate::models::Run patch_run(id, run_patch)
Modifies run

Updates run according to given arguments  On success, new run state is returned

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** |  | [required] |
**run_patch** | Option<[**RunPatch**](RunPatch.md)> |  |  |

### Return type

[**crate::models::Run**](Run.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## submit_run

> crate::models::Run submit_run(run_simple_submit_params)
Submit run

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**run_simple_submit_params** | [**RunSimpleSubmitParams**](RunSimpleSubmitParams.md) |  | [required] |

### Return type

[**crate::models::Run**](Run.md)

### Authorization

[AccessToken](../README.md#AccessToken)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

