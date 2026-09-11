#pragma once
#include <QByteArray>
#include <QString>
#include <QVector>

namespace studio {
struct Property { int line = -1; QString key; QString value; bool multiline = false; };
struct Node {
    int line = 0, end = 0, parent = -1, indent = 0;
    QString name;
    bool widget = true;
    QVector<Property> properties;
};
struct Issue { int line; QString message; };
class Document {
public:
    static constexpr qint64 MaxBytes = 8 * 1024 * 1024;
    bool load(const QString& path, QString& error);
    bool parse(const QByteArray& bytes, QString& error);
    QByteArray bytes() const;
    const QVector<Node>& nodes() const { return nodes_; }
    const QVector<Issue>& issues() const { return issues_; }
    QString value(int node, const QString& key, const QString& fallback = {}) const;
    bool setProperty(int node, const QString& key, const QString& value, QString& error);
    bool insertWidget(int parent, const QString& type, const QString& id, QString& error);
    bool removeNode(int node, QString& error);
    bool save(const QString& path, bool overwrite, QString& error);
    QString path() const { return path_; }
    QString encoding() const { return utf8_ ? QStringLiteral("UTF-8") : QStringLiteral("Windows-1252"); }
    QString text() const;
    bool replaceText(const QString& text, QString& error);
    bool diskChanged() const;
private:
    struct Line { QByteArray content; QByteArray ending; };
    QVector<Line> lines_;
    QVector<Node> nodes_;
    QVector<Issue> issues_;
    QByteArray bom_, savedBytes_;
    QString path_;
    bool utf8_ = true;
    void index();
    QString decode(const QByteArray& text) const;
    QByteArray encode(const QString& text, bool& ok) const;
};
}
