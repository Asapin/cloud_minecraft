## Create new account

*You can skip this part if you already have Azure account*

Open `https://portal.azure.com/` and create a new account. When you finish, you should see a `Start with an Azure free trial` banner.

* Click `Start` under `Start with an Azure free trial` banner
* Click `Start free`
* Enter your name, phone number and the address
* Verify your phone number
* Click `Next`
* Enter you credit card details (you might be charged a small amount to verify the validity of your card)

## Create Resource group

Once you have an account, you need to create new resource group. A resource group is a container, that holds related resources.

* Open `https://portal.azure.com/`
* Click `Resource groups`
* Click `Create`
* `Subscription` - all resources within subscription are billed together. If you have created new account, you should have only one subscription. Leave it as is
* `Resource group` - the name of the resource group you want to create. You can enter something like `minecraft-rg`
* `Region` - select the region where you want your Minecraft server to be located. **The price of the resources and the ping to Minecraft server is different depending on the region**
  * Use [Cloud ping test](https://cloudpingtest.com/azure) site to check your ping to various Azure regions
  * Use [this Azure page](https://azure.microsoft.com/en-us/pricing/details/container-instances/) to check resource price in each region
* Click `Review + create`
* Wait for the validation to finish and click `Create` again

## Creating server

Now you can start creating Minecraft server.

* Open `https://portal.azure.com/#create/Microsoft.Template`
* Click `Build your own template in the editor`
* Copy the content of [azure.json](azure.json) into the editor
* Click `Save`
* `Subscription` - select which subscription account to use. If you have created new account, you should have only one subscription
* `Resource group` - select resource group that you created in the previous step
* `Region` - after you have selected resource group name, it should've automatically changed to the same region as the resource group
* `Storage name` - enter the name for you storage account. The name should be unique across Azure
* `Server name` - enter the name for you Minecraft server. This name will also be used to generate the address of the server
* `Cpu` - How many cores to allocate for the server
* `Memory` - How much RAM (in Gb) to allocate for the server
* Fill in the rest of options
* Click `Review + create`
* Wait for the validation to finish and click `Create` again

After you click `Create` you will be redirected to another page where you can see the progress of the deployment. After the deployment has finished, click `Outputs` on the left. There you'll see the address of your server. Use this address to access both admin panel and Minecraft server itself.