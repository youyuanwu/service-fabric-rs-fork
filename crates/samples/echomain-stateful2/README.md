# Stateful app
This sample shows how can you create stateful Service Fabric application using bridges and proxies instead of using IFabric interfaces directly.

# Manual tests
```ps1
Update-ServiceFabricService -Stateful fabric:/StatefulEchoApp/StatefulEchoAppService -PartitionNamesToAdd @("C","D")

Resolve-ServiceFabricService -ServiceName fabric:/StatefulEchoApp/StatefulEchoAppService -PartitionKindNamed -PartitionKey A -ForceRefresh
```
TODO: how to pass init data for new replica?