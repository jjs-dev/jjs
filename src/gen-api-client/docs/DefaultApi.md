# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_version**](DefaultApi.md#api_version) | **GET** /system/api-version | Route Api Version
[**is_dev**](DefaultApi.md#is_dev) | **GET** /system/is-dev | Route Is Dev



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

