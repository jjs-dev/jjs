# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_version**](DefaultApi.md#api_version) | **GET** /system/api-version | Route Api Version
[**is_dev**](DefaultApi.md#is_dev) | **GET** /system/is-dev | Route Is Dev
[**list_runs**](DefaultApi.md#list_runs) | **GET** /runs | Route List Runs
[**submit_run**](DefaultApi.md#submit_run) | **POST** /runs | Route Submit



## api_version

> crate::models::ApiVersion api_version()
Route Api Version

Returns API version  Version is returned in format {major: MAJOR, minor: MINOR}. MAJOR component is incremented, when backwards-incompatible changes were made. MINOR component is incremented, when backwards-compatible changes were made.  It means, that if you tested application with apiVersion == X.Y, your application should assert that MAJOR = X and MINOR >= Y

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::ApiVersion**](ApiVersion.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## is_dev

> bool is_dev()
Route Is Dev

Returns if JJS is running in development mode.  Please note that you don't have to respect this information, but following is recommended: 1. Display it in each page/view. 2. Change theme. 3. On login view, add button \"login as root\".

### Parameters

This endpoint does not need any parameter.

### Return type

**bool**

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_runs

> Vec<crate::models::Run> list_runs()
Route List Runs

Lists runs  This operation returns all created runs

### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<crate::models::Run>**](Run.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## submit_run

> crate::models::Run submit_run(run_submit_simple_params)
Route Submit

Submits new run  This operation creates new run, with given source code, and queues it for judging. Created run will be returned. All fields against `id` will match fields of request body; `id` will be real id of this run.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**run_submit_simple_params** | [**RunSubmitSimpleParams**](RunSubmitSimpleParams.md) |  | [required] |

### Return type

[**crate::models::Run**](Run.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

