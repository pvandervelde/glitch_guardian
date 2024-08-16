#
# ENVIRONMENT
#

variable "category" {
    default = "glitchg"
    description = "The name of the category that all the resources are running in."
}

variable "environment" {
    default = "production"
    description = "The name of the environment that all the resources are running in."
}

#
# GITHUB
#

variable "github_token" {
  description = "GitHub Personal Access Token"
  type        = string
}

variable "github_project_id" {
  description = "The ID of the GitHub project that the issues are being created in."
  type        = string
}

variable "github_webhook_secret" {
  description = "The secret used to validate the GitHub webhook."
  type        = string
}

#
# LOCATION
#

variable "location" {
    default = "australiaeast"
    description = "The full name of the Azure region in which the resources should be created."
}

#
# META
#

variable "meta_git_sha" {
    description = "The commit ID of the current commit from which the plan is being created."
    type = string
}

variable "meta_version" {
    description = "The version of the infrastructure as it is being generated."
    type = string
}

#
# TAGS
#

variable "tags" {
  description = "Tags to apply to all resources created."
  type = map(string)
  default = { }
}
