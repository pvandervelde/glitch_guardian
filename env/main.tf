terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.115"
    }
  }

  backend "azurerm" {
    resource_group_name  = "p-aue-tf-tfstate-rg"
    storage_account_name = "paueterraformstate"
    container_name       = "p-aue-tf-tfstate-sc"
    key                  = "prod.glitchg.tfstate"
  }
}

provider "azurerm" {
  features {}
}

#
# LOCALS
#

locals {
    location_map = {
        australiacentral = "auc",
        australiacentral2 = "auc2",
        australiaeast = "aue",
        australiasoutheast = "ause",
        brazilsouth = "brs",
        canadacentral = "cac",
        canadaeast = "cae",
        centralindia = "inc",
        centralus = "usc",
        eastasia = "ase",
        eastus = "use",
        eastus2 = "use2",
        francecentral = "frc",
        francesouth = "frs",
        germanynorth = "den",
        germanywestcentral = "dewc",
        japaneast = "jpe",
        japanwest = "jpw",
        koreacentral = "krc",
        koreasouth = "kre",
        northcentralus = "usnc",
        northeurope = "eun",
        norwayeast = "noe",
        norwaywest = "now",
        southafricanorth = "zan",
        southafricawest = "zaw",
        southcentralus = "ussc",
        southeastasia = "asse",
        southindia = "ins",
        switzerlandnorth = "chn",
        switzerlandwest = "chw",
        uaecentral = "aec",
        uaenorth = "aen",
        uksouth = "uks",
        ukwest = "ukw",
        westcentralus = "uswc",
        westeurope = "euw",
        westindia = "inw",
        westus = "usw",
        westus2 = "usw2",
    }
}

locals {
    environment_short = substr(var.environment, 0, 1)
    location_short = lookup(local.location_map, var.location, "aue")
}

# Name prefixes
locals {
    name_prefix = "${local.environment_short}-${local.location_short}"
    name_prefix_tf = "${local.name_prefix}-tf-${var.category}"
}

locals {
    common_tags = {
        category    = "${var.category}"
        environment = "${var.environment}"
        location    = "${var.location}"
        git_sha  = "${var.meta_git_sha}"
        version = "${var.meta_version}"
    }

    extra_tags = {
    }
}

#
# Data
#

data "http" "github_meta" {
    url = "https://api.github.com/meta"
}

data "azurerm_client_config" "current" {}

locals {
    github_hook_ips = jsondecode(data.http.github_meta.response_body).hooks
}

#
# Resource group
#

resource "azurerm_resource_group" "rg" {
  location = "${var.location}"
  name     = "${local.name_prefix_tf}-rg"

  tags = merge(
    local.common_tags,
    local.extra_tags,
    var.tags,
    {
        "purpose" = "${var.category}"
    } )
}

#
# Storage accounts
#

resource "azurerm_storage_account" "sa" {
  name                     = "${local.environment_short}${local.location_short}tf${var.category}sa"
  resource_group_name      = azurerm_resource_group.rg.name
  location                 = azurerm_resource_group.rg.location
  account_tier             = "Standard"
  account_replication_type = "LRS"

  tags = merge(
    local.common_tags,
    local.extra_tags,
    var.tags,
    {
        "purpose" = "${var.category}"
    } )
}

#
# Key vault
#

resource "azurerm_key_vault" "kv" {
    name = "${local.name_prefix_tf}-tf-${var.category}-kv"
    location = azurerm_resource_group.rg.location
    resource_group_name = azurerm_resource_group.rg.name
    enabled_for_disk_encryption = true
    tenant_id = data.azurerm_client_config.current.tenant_id
    soft_delete_retention_days = 7
    purge_protection_enabled = false

    sku_name = "standard"

    access_policy {
        tenant_id = data.azurerm_client_config.current.tenant_id
        object_id = data.azurerm_client_config.current.object_id

        key_permissions = [
            "Get",
        ]

        secret_permissions = [
            "Get",
            "List",
            "Set",
            "Delete",
            "Purge",
        ]

        storage_permissions = [
            "Get",
        ]
    }

    tags = merge(
        local.common_tags,
        local.extra_tags,
        var.tags,
        {
            "purpose" = "${var.category}"
        } )
}

resource "azurerm_key_vault_secret" "github_token" {
    name = "GithubToken"
    value = var.github_token
    key_vault_id = azurerm_key_vault.kv.id
}

resource "azurerm_key_vault_secret" "github_webhook_secret" {
    name = "GithubWebhookSecret"
    value = var.github_webhook_secret
    key_vault_id = azurerm_key_vault.kv.id
}

#
# Function app
#

resource "azurerm_service_plan" "asp" {
  name                = "${local.name_prefix}-tf-${var.category}-asp"
  location            = azurerm_resource_group.rg.location
  resource_group_name = azurerm_resource_group.rg.name
  os_type             = "Linux"
  sku_name            = "Y1"

  tags = merge(
    local.common_tags,
    local.extra_tags,
    var.tags)
}

resource "azurerm_linux_function_app" "fa" {
  name                       = "${local.name_prefix}-tf-${var.category}-function"
  location                   = azurerm_resource_group.rg.location
  resource_group_name        = azurerm_resource_group.rg.name

  service_plan_id            = azurerm_service_plan.asp.id
  storage_account_name       = azurerm_storage_account.sa.name
  storage_account_access_key = azurerm_storage_account.sa.primary_access_key

  identity {
    type = "SystemAssigned"
  }

  app_settings = {
    "FUNCTIONS_WORKER_RUNTIME" = "custom"
    "GITHUB_TOKEN"             = var.github_token
    PROJECT_ID                 = var.github_project_id
  }

  site_config {

  }

  # site_config {
  #   ip_restriction = concat([
  #       for ip in local.github_hook_ips : {
  #           ip_address = ip
  #           action     = "Allow"
  #           priority = 100
  #           name = "Allow Github"
  #       }
  #   ], [
  #       {
  #           ip_address = "0.0.0.0/0"
  #           action     = "Deny"
  #           priority = 200
  #           name = "Deny All"
  #       }
  #   ])
  # }

  tags = merge(
    local.common_tags,
    local.extra_tags,
    var.tags)
}

resource "azurerm_key_vault_access_policy" "function_app" {
    key_vault_id = azurerm_key_vault.kv.id
    tenant_id = data.azurerm_client_config.current.tenant_id
    object_id = azurerm_function_app.fa.identity[0].principal_id

    secret_permissions = [
        "Get",
    ]
}
