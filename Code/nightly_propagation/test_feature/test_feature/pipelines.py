# Define your item pipelines here
#
# Don't forget to add your pipeline to the ITEM_PIPELINES setting
# See: https://docs.scrapy.org/en/latest/topics/item-pipeline.html


# useful for handling different item types with a single interface
from itemadapter import ItemAdapter


class FeaturePipeline:
    def __init__(self) -> None:
        self.all_features = []

    def open_spider(self, spider):
        pass

    def close_spider(self, spider):
        file = open('./features.txt', 'w+')
        for item in self.all_features:
            file.write("%s\n" % item)
        file.close()

    def process_item(self, item, spider):
        self.all_features.append(item['name'])
        return item
