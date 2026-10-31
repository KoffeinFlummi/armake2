class CfgPatches {
    class g {
        units[] = {};
        requiredAddons[] = {"DZ_Data"};
        requiredVersion = 0.1;
    };
};
class CfgVehicles {
    class Inventory_Base;
    class G_Item: Inventory_Base {
        scope = 2;
        displayName = "G item";
        weight = 150;
        itemSize[] = {2, 3};
        hiddenSelectionsTextures[] = {"g\data\g_co.paa"};
        class Foo {};
    };
    class Bar {};
};
