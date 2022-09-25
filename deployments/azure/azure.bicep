@minLength(3)
@maxLength(24)
@description('Provide a name for the storage account. Use only lower case letters and numbers. The name must be unique across Azure.')
param storageName string

@minLength(3)
@description('Provide a server name for Minecraft server. The address of the server will be "{server_name}.{region}.azurecontainer.io"')
param serverName string

@allowed([
  'asapin/cloud_minecraft:1.19.2-fabric'
  'asapin/cloud_minecraft:1.19.2-better-mc'
  'asapin/cloud_minecraft:1.19.2-medieval-mc'
])
@description('Which version of Minecraft server you want to deploy')
param imageName string = 'asapin/cloud_minecraft:1.19.2-fabric'

@minValue(1)
@maxValue(4)
@description('CPU limit')
param cpu int = 2

@minValue(1)
@maxValue(8)
@description('Memory limit in GB')
param memory int = 4

@description('Username for the admin panel')
param adminUsername string

@secure()
@description('Password for the admin panel')
param adminPassword string

@description('Whether you accept Minecraft EULA or not. Must be set to true to start the server')
param eula bool

@allowed([
  'peaceful'
  'easy'
  'normal'
  'hard'
])
@description('Server difficulty')
param difficulty string = 'normal'

@description('Whether the hardcore mode is on or off')
param hardcore bool = false

@minValue(1)
@maxValue(255)
@description('The maximum number of players that can on the server at the same time')
param maxPlayers int = 10

@minValue(128)
@maxValue(65535)
@description('The maximum possible radius of the world in blocks. The actual world will be two times bigger than this value')
param maxWorldRadius int = 1000

@description('Message of the day')
param motd string = 'Minecraft on demand'

@minValue(1)
@maxValue(255)
@description('Players are kicked from the server if they are idle for more than that many minutes')
param playerIdleTimeout int = 10

@minValue(1)
@maxValue(255)
@description('Server will automatically shutdown, if there are now players for more than that many minutes')
param serverIdleTimeout int = 10

@minValue(1)
@maxValue(255)
@description('The amount of visible chunks in each direction')
param viewDistance int = 10

@description('Enable PvP on the server')
param pvp bool = true

resource storageAccount 'Microsoft.Storage/storageAccounts@2022-05-01' = {
  name: storageName
  location: resourceGroup().location
  sku: {
    name: 'Standard_LRS'
  }
  kind: 'StorageV2'
  properties: {
    accessTier: 'Hot'
    allowBlobPublicAccess: false
    allowCrossTenantReplication: false
    allowedCopyScope: 'AAD'
    allowSharedKeyAccess: true
    defaultToOAuthAuthentication: false
    isHnsEnabled: true
    isLocalUserEnabled: false
    isNfsV3Enabled: false
    isSftpEnabled: false
    largeFileSharesState: 'Disabled'
    minimumTlsVersion: 'TLS1_2'
    // publicNetworkAccess: 'Disabled'
    routingPreference: {
      publishInternetEndpoints: false
      publishMicrosoftEndpoints: false
      routingChoice: 'MicrosoftRouting'
    }
    supportsHttpsTrafficOnly: true
  }
}

resource fileService 'Microsoft.Storage/storageAccounts/fileServices@2022-05-01' = {
  name: 'default'
  parent: storageAccount
}

resource mcWorldData 'Microsoft.Storage/storageAccounts/fileServices/shares@2022-05-01' = {
  name: 'world-data'
  parent: fileService
  properties: {
    accessTier: 'Hot'
    shareQuota: 5
  }
}

resource container 'Microsoft.ContainerInstance/containerGroups@2021-10-01' = {
  name: 'mc-container'
  location: resourceGroup().location
  properties: {
    containers: [
      {
        name: 'mc-server'
        properties: {
          image: imageName
          resources: {
            requests: {
              cpu: cpu
              memoryInGB: memory
            }
            limits: {
              cpu: cpu
              memoryInGB: memory
            }
          }
          environmentVariables: [
            {
              name: 'ADMIN_USERNAME'
              value: adminUsername
            }
            {
              name: 'ADMIN_PASSWORD'
              value: adminPassword
            }
            {
              name: 'EULA'
              value: string(eula)
            }
            {
              name: 'DIFFICULTY'
              value: difficulty
            }
            {
              name: 'HARDCORE'
              value: string(hardcore)
            }
            {
              name: 'MAX_PLAYERS'
              value: string(maxPlayers)
            }
            {
              name: 'MAX_WORLD_RADIUS'
              value: string(maxWorldRadius)
            }
            {
              name: 'MOTD'
              value: motd
            }
            {
              name: 'PLAYER_IDLE_TIMEOUT'
              value: string(playerIdleTimeout)
            }
            {
              name: 'SERVER_IDLE_TIMEOUT'
              value: string(serverIdleTimeout)
            }
            {
              name: 'VIEW_DISTANCE'
              value: string(viewDistance)
            }
            {
              name: 'PVP'
              value: string(pvp)
            }
          ]
          ports: [
            {
              port: 80
              protocol: 'TCP'
            }
            {
              port: 25565
              protocol: 'TCP'
            }
          ]
          volumeMounts: [
            {
              name: 'world-data'
              mountPath: '/data'
              readOnly: false
            }
          ]
        }
      }
    ]
    osType: 'Linux'
    volumes: [
      {
        name: 'world-data'
        azureFile: {
          shareName: mcWorldData.name
          storageAccountName: storageAccount.name
          readOnly: false
          storageAccountKey: storageAccount.listKeys().keys[0].value
        }
      }
    ]
    restartPolicy: 'Never'
    ipAddress: {
      ports: [
        {
          port: 80
          protocol: 'TCP'
        }
        {
          port: 25565
          protocol: 'TCP'
        }
      ]
      type: 'Public'
      autoGeneratedDomainNameLabelScope: 'TenantReuse'
      dnsNameLabel: serverName
    }
  }
}

output hostname string = container.properties.ipAddress.fqdn
